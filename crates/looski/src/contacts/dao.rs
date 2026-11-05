// SPDX-License-Identifier: AGPL-3.0-only
use super::{Contact, ContactIdentifier, ContactInput, Error, Identifier, ListContacts};
use crate::access::Access;
use sqlx::{PgConnection, Row, postgres::PgRow, types::Json};
use uuid::Uuid;

// SQL fragments below are constants; all caller values use bind parameters.
const COLUMNS: &str = "c.properties,c.id,c.display_name,c.notes,c.owner_membership_id,c.archived_at IS NOT NULL AS archived,c.merged_into_id,c.version,c.created_at::text,c.updated_at::text,
 COALESCE((SELECT jsonb_agg(jsonb_build_object('kind',i.kind,'normalized_value',i.normalized_value,'value',ci.original_value,'country',ci.country,'extension',ci.extension) ORDER BY i.kind,ci.position,i.normalized_value)
 FROM contacts.contact_identifiers ci JOIN contacts.identifiers i ON (i.workspace_id,i.id)=(ci.workspace_id,ci.identifier_id)
 WHERE ci.workspace_id=c.workspace_id AND ci.contact_id=c.id AND ci.ended_at IS NULL),'[]'::jsonb) AS identifiers";
fn decode(row: PgRow) -> Result<Contact, Error> {
    Ok(Contact {
        properties: row
            .try_get::<sqlx::types::Json<serde_json::Map<String, serde_json::Value>>, _>(
                "properties",
            )?
            .0,
        id: row.try_get("id")?,
        display_name: row.try_get("display_name")?,
        notes: row.try_get("notes")?,
        owner_membership_id: row.try_get("owner_membership_id")?,
        archived: row.try_get("archived")?,
        merged_into_id: row.try_get("merged_into_id")?,
        version: row.try_get("version")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        identifiers: row
            .try_get::<Json<Vec<ContactIdentifier>>, _>("identifiers")?
            .0,
    })
}
pub(super) async fn get(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    access: &Access,
    lock: bool,
) -> Result<Contact, Error> {
    let sql = format!(
        "SELECT {COLUMNS} FROM contacts.contacts c WHERE c.workspace_id=$1 AND c.id=$2 AND ($3 OR c.owner_membership_id=$4) {}",
        if lock { "FOR UPDATE OF c" } else { "" }
    );
    decode(
        sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(id)
            .bind(access.workspace_wide)
            .bind(access.membership_id)
            .fetch_optional(db)
            .await?
            .ok_or(Error::NotFound)?,
    )
}
pub(super) async fn list(
    db: &mut PgConnection,
    workspace: Uuid,
    access: &Access,
    query: &ListContacts,
    limit: i64,
) -> Result<Vec<Contact>, Error> {
    if let Some(cursor) = query.after {
        get(db, workspace, cursor, access, false).await?;
    }
    let fields = crate::records::definitions(db, workspace, "contacts").await?;
    let filter = query
        .filter
        .as_ref()
        .map(|s| {
            if s.len() > 16384 {
                return Err(Error::Invalid("Filter is too large"));
            }
            serde_json::from_str::<crate::records::Filter>(s)
                .map_err(|_| Error::Invalid("Invalid filter expression"))
        })
        .transpose()?;
    if let Some(f) = &filter {
        crate::records::validate_filter(f, &fields)?;
    }
    let columns = COLUMNS.replace("c.", "r.");
    let mut sql = sqlx::QueryBuilder::<sqlx::Postgres>::new(format!(
        "SELECT {columns} FROM contacts.contacts r WHERE r.workspace_id="
    ));
    sql.push_bind(workspace)
        .push(" AND (")
        .push_bind(access.workspace_wide)
        .push(" OR r.owner_membership_id=")
        .push_bind(access.membership_id)
        .push(") AND (r.archived_at IS NOT NULL)=")
        .push_bind(query.archived.unwrap_or(false));
    sql.push(" AND r.merged_into_id IS NULL");
    if let Some(search) = &query.search {
        sql.push(" AND strpos(lower(r.display_name),lower(")
            .push_bind(search.clone())
            .push("))>0");
    }
    if let Some(tag) = query.tag_id {
        sql.push(" AND EXISTS(SELECT 1 FROM contacts.contact_tags t WHERE t.workspace_id=r.workspace_id AND t.contact_id=r.id AND t.tag_id=").push_bind(tag).push(")");
    }
    if let Some(f) = &filter {
        sql.push(" AND ");
        crate::records::append_filter(&mut sql, "contacts", &fields, f);
    }
    crate::records::append_sort(&mut sql, "contacts", &fields, query, workspace)?;
    sql.push(" LIMIT ").push_bind(limit);
    sql.build()
        .fetch_all(db)
        .await?
        .into_iter()
        .map(decode)
        .collect()
}

pub(super) async fn lock_identifiers(
    db: &mut PgConnection,
    workspace: Uuid,
    ids: &[Identifier],
) -> Result<(), Error> {
    // Suggestion decisions use the database's same lower() as candidate lookup.
    // These locks serialize review decisions; exact registry identities stay case-sensitive.
    let emails: Vec<&str> = ids
        .iter()
        .filter(|i| i.kind == "email")
        .map(|i| i.normalized.as_str())
        .collect();
    let folded = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT lower(value) FROM unnest($1::text[]) AS value ORDER BY 1",
    )
    .bind(emails)
    .fetch_all(&mut *db)
    .await?;
    for key in folded {
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
            .bind(format!("identifier-case:{workspace}:{key}"))
            .execute(&mut *db)
            .await?;
    }
    // Shared namespace and sorted keys serialize even identifiers not inserted yet.
    let mut ordered: Vec<_> = ids.iter().collect();
    ordered.sort_by_key(|i| (&i.kind, &i.normalized));
    for item in ordered {
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
            .bind(format!(
                "identifier:{workspace}:{}:{}",
                item.kind, item.normalized
            ))
            .execute(&mut *db)
            .await?;
    }
    Ok(())
}
pub(super) async fn matches(
    db: &mut PgConnection,
    workspace: Uuid,
    ids: &[Identifier],
) -> Result<Vec<Uuid>, Error> {
    let mut matches = Vec::new();
    for item in ids {
        let rows=sqlx::query_scalar::<_,Uuid>("SELECT ci.contact_id FROM contacts.identifiers i JOIN contacts.contact_identifiers ci ON (ci.workspace_id,ci.identifier_id)=(i.workspace_id,i.id) WHERE i.workspace_id=$1 AND i.kind=$2 AND i.normalized_value=$3 AND ci.ended_at IS NULL")
        .bind(workspace).bind(&item.kind).bind(&item.normalized).fetch_all(&mut *db).await?;
        matches.extend(rows);
    }
    matches.sort();
    matches.dedup();
    Ok(matches)
}
pub(super) async fn insert(
    db: &mut PgConnection,
    workspace: Uuid,
    owner: Uuid,
    input: &ContactInput,
) -> Result<Uuid, Error> {
    Ok(sqlx::query_scalar("INSERT INTO contacts.contacts(workspace_id,display_name,notes,owner_membership_id,properties) VALUES($1,$2,$3,$4,$5) RETURNING id")
    .bind(workspace).bind(input.display_name.trim()).bind(&input.notes).bind(owner).bind(sqlx::types::Json(input.properties.clone().unwrap_or_default())).fetch_one(db).await?)
}
pub(super) async fn update(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    owner: Uuid,
    input: &ContactInput,
) -> Result<(), Error> {
    sqlx::query("UPDATE contacts.contacts SET display_name=$3,notes=$4,owner_membership_id=$5,properties=COALESCE($6,properties),version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
    .bind(workspace).bind(id).bind(input.display_name.trim()).bind(&input.notes).bind(owner).bind(input.properties.as_ref().map(sqlx::types::Json)).execute(db).await?;
    Ok(())
}
pub(super) async fn archive(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    archived: bool,
) -> Result<(), Error> {
    sqlx::query("UPDATE contacts.contacts SET archived_at=CASE WHEN $3 THEN now() ELSE NULL END,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2")
    .bind(workspace).bind(id).bind(archived).execute(db).await?;
    Ok(())
}
pub(super) async fn set_identifiers(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    ids: &[Identifier],
) -> Result<(), Error> {
    // History is retained when an association ends, including contact edits.
    sqlx::query("UPDATE contacts.contact_identifiers SET ended_at=now() WHERE workspace_id=$1 AND contact_id=$2 AND ended_at IS NULL").bind(workspace).bind(id).execute(&mut *db).await?;
    for (position, item) in ids.iter().enumerate() {
        sqlx::query("INSERT INTO contacts.identifiers(workspace_id,kind,normalized_value) VALUES($1,$2,$3) ON CONFLICT DO NOTHING")
        .bind(workspace).bind(&item.kind).bind(&item.normalized).execute(&mut *db).await?;
        sqlx::query("INSERT INTO contacts.contact_identifiers(workspace_id,contact_id,identifier_id,original_value,country,extension,position) SELECT $1,$2,id,$5,$6,$7,$8 FROM contacts.identifiers WHERE workspace_id=$1 AND kind=$3 AND normalized_value=$4")
        .bind(workspace).bind(id).bind(&item.kind).bind(&item.normalized).bind(&item.original).bind(&item.country).bind(&item.extension).bind(position as i32).execute(&mut *db).await?;
    }
    Ok(())
}
pub(super) async fn lock_merge(db: &mut PgConnection, workspace: Uuid) -> Result<(), Error> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(format!("contact-merge:{workspace}"))
        .execute(db)
        .await?;
    Ok(())
}
pub(super) async fn merge_into(
    db: &mut PgConnection,
    workspace: Uuid,
    source: Uuid,
    survivor: Uuid,
) -> Result<(), Error> {
    sqlx::query("UPDATE contacts.contact_identifiers SET ended_at=now() WHERE workspace_id=$1 AND contact_id=$2 AND ended_at IS NULL").bind(workspace).bind(source).execute(&mut *db).await?;
    sqlx::query("UPDATE contacts.contacts SET merged_into_id=$3,archived_at=COALESCE(archived_at,now()),version=version+1,updated_at=now() WHERE workspace_id=$1 AND (id=$2 OR merged_into_id=$2)").bind(workspace).bind(source).bind(survivor).execute(db).await?;
    Ok(())
}

pub(super) async fn touch(db: &mut PgConnection, workspace: Uuid, id: Uuid) -> Result<(), Error> {
    sqlx::query("UPDATE contacts.contacts SET version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2").bind(workspace).bind(id).execute(db).await?;
    Ok(())
}
