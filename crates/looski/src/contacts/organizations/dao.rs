// SPDX-License-Identifier: AGPL-3.0-only
use super::{Organization, OrganizationInput, Relationship, RelationshipInput};
use crate::{
    access::Access,
    contacts::{Error, ListContacts},
};
use sqlx::{PgConnection, Row, postgres::PgRow};
use uuid::Uuid;
const COLUMNS: &str = r#"o.id,o.name,o.website_url,o.notes,o.owner_membership_id,o.archived_at IS NOT NULL AS archived,o.version,
 o.created_at::text,o.updated_at::text"#;
fn decode(row: PgRow) -> Result<Organization, Error> {
    Ok(Organization {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        website_url: row.try_get("website_url")?,
        notes: row.try_get("notes")?,
        owner_membership_id: row.try_get("owner_membership_id")?,
        archived: row.try_get("archived")?,
        version: row.try_get("version")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}
pub(super) async fn get(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    access: &Access,
    lock: bool,
) -> Result<Organization, Error> {
    // Only fixed SQL fragments are interpolated; every caller value is bound.
    let sql = format!(
        "SELECT {COLUMNS} FROM contacts.organizations o WHERE o.workspace_id=$1 AND o.id=$2 AND ($3 OR o.owner_membership_id=$4) {}",
        if lock { "FOR UPDATE OF o" } else { "" }
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
) -> Result<Vec<Organization>, Error> {
    if let Some(cursor) = query.after {
        get(db, workspace, cursor, access, false).await?;
    }
    let sql = format!(
        "SELECT {COLUMNS} FROM contacts.organizations o WHERE o.workspace_id=$1 AND ($2 OR o.owner_membership_id=$3) AND (o.archived_at IS NOT NULL)=$4 AND ($5::uuid IS NULL OR (o.created_at,o.id)>(SELECT created_at,id FROM contacts.organizations WHERE workspace_id=$1 AND id=$5)) AND ($6::text IS NULL OR strpos(lower(o.name),lower($6))>0) ORDER BY o.created_at,o.id LIMIT $7"
    );
    sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(workspace)
        .bind(access.workspace_wide)
        .bind(access.membership_id)
        .bind(query.archived.unwrap_or(false))
        .bind(query.after)
        .bind(&query.search)
        .bind(limit)
        .fetch_all(db)
        .await?
        .into_iter()
        .map(decode)
        .collect()
}
pub(super) async fn insert(
    db: &mut PgConnection,
    workspace: Uuid,
    owner: Uuid,
    input: &OrganizationInput,
) -> Result<Uuid, Error> {
    Ok(sqlx::query_scalar("INSERT INTO contacts.organizations(workspace_id,name,website_url,notes,owner_membership_id) VALUES($1,$2,$3,$4,$5) RETURNING id").bind(workspace).bind(input.name.trim()).bind(&input.website_url).bind(&input.notes).bind(owner).fetch_one(db).await?)
}
pub(super) async fn update(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    owner: Uuid,
    input: &OrganizationInput,
) -> Result<(), Error> {
    sqlx::query("UPDATE contacts.organizations SET name=$3,website_url=$4,notes=$5,owner_membership_id=$6,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2").bind(workspace).bind(id).bind(input.name.trim()).bind(&input.website_url).bind(&input.notes).bind(owner).execute(db).await?;
    Ok(())
}
pub(super) async fn archive(
    db: &mut PgConnection,
    workspace: Uuid,
    id: Uuid,
    archived: bool,
) -> Result<(), Error> {
    sqlx::query("UPDATE contacts.organizations SET archived_at=CASE WHEN $3 THEN now() ELSE NULL END,version=version+1,updated_at=now() WHERE workspace_id=$1 AND id=$2").bind(workspace).bind(id).bind(archived).execute(db).await?;
    Ok(())
}
const RELATION_COLUMNS: &str = r#"r.id,r.contact_id,r.organization_id,o.name AS organization_name,r.relationship_label,r.job_title,r.is_primary,r.ended_at IS NOT NULL AS ended,r.created_at::text"#;
fn decode_relation(row: PgRow) -> Result<Relationship, Error> {
    Ok(Relationship {
        id: row.try_get("id")?,
        contact_id: row.try_get("contact_id")?,
        organization_id: row.try_get("organization_id")?,
        organization_name: row.try_get("organization_name")?,
        relationship_label: row.try_get("relationship_label")?,
        job_title: row.try_get("job_title")?,
        is_primary: row.try_get("is_primary")?,
        ended: row.try_get("ended")?,
        created_at: row.try_get("created_at")?,
    })
}
pub(super) async fn relationships(
    db: &mut PgConnection,
    workspace: Uuid,
    contact: Uuid,
    access: &Access,
) -> Result<Vec<Relationship>, Error> {
    let sql = format!(
        "SELECT {RELATION_COLUMNS} FROM contacts.contact_organizations r JOIN contacts.organizations o ON (o.workspace_id,o.id)=(r.workspace_id,r.organization_id) WHERE r.workspace_id=$1 AND r.contact_id=$2 AND r.ended_at IS NULL AND ($3 OR o.owner_membership_id=$4) ORDER BY r.is_primary DESC,o.name,o.id LIMIT 100"
    );
    sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(workspace)
        .bind(contact)
        .bind(access.workspace_wide)
        .bind(access.membership_id)
        .fetch_all(db)
        .await?
        .into_iter()
        .map(decode_relation)
        .collect()
}
pub(super) async fn relate(
    db: &mut PgConnection,
    workspace: Uuid,
    contact: Uuid,
    organization: Uuid,
    input: &RelationshipInput,
) -> Result<Relationship, Error> {
    let previous=sqlx::query_scalar::<_,Uuid>("SELECT id FROM contacts.contact_organizations WHERE workspace_id=$1 AND contact_id=$2 AND organization_id=$3 AND ended_at IS NULL").bind(workspace).bind(contact).bind(organization).fetch_optional(&mut *db).await?;
    if previous.is_none() && input.ended {
        return Err(Error::NotFound);
    }
    if previous.is_none() && sqlx::query_scalar::<_,i64>("SELECT count(*) FROM contacts.contact_organizations WHERE workspace_id=$1 AND contact_id=$2 AND ended_at IS NULL").bind(workspace).bind(contact).fetch_one(&mut *db).await?>=100 {return Err(Error::Conflict("A contact supports at most 100 current organization relationships"));}
    sqlx::query("UPDATE contacts.contact_organizations SET ended_at=now() WHERE workspace_id=$1 AND contact_id=$2 AND organization_id=$3 AND ended_at IS NULL").bind(workspace).bind(contact).bind(organization).execute(&mut *db).await?;
    let id = if input.ended {
        previous.ok_or(Error::NotFound)?
    } else {
        if input.is_primary {
            // Preserve the previous primary relationship version instead of overwriting history.
            sqlx::query("WITH old AS (UPDATE contacts.contact_organizations SET ended_at=now() WHERE workspace_id=$1 AND contact_id=$2 AND is_primary AND ended_at IS NULL RETURNING *) INSERT INTO contacts.contact_organizations(workspace_id,contact_id,organization_id,relationship_label,job_title,is_primary) SELECT workspace_id,contact_id,organization_id,relationship_label,job_title,false FROM old").bind(workspace).bind(contact).execute(&mut *db).await?;
        }
        sqlx::query_scalar::<_,Uuid>("INSERT INTO contacts.contact_organizations(workspace_id,contact_id,organization_id,relationship_label,job_title,is_primary) VALUES($1,$2,$3,$4,$5,$6) RETURNING id").bind(workspace).bind(contact).bind(organization).bind(input.relationship_label.trim()).bind(input.job_title.trim()).bind(input.is_primary).fetch_one(&mut *db).await?
    };
    let sql = format!(
        "SELECT {RELATION_COLUMNS} FROM contacts.contact_organizations r JOIN contacts.organizations o ON (o.workspace_id,o.id)=(r.workspace_id,r.organization_id) WHERE r.workspace_id=$1 AND r.id=$2"
    );
    decode_relation(
        sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(workspace)
            .bind(id)
            .fetch_one(db)
            .await?,
    )
}
pub(super) async fn merge(
    db: &mut PgConnection,
    workspace: Uuid,
    source: Uuid,
    survivor: Uuid,
) -> Result<(), Error> {
    let conflict=sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM contacts.contact_organizations s JOIN contacts.contact_organizations t ON t.workspace_id=s.workspace_id AND t.contact_id=$3 AND t.ended_at IS NULL WHERE s.workspace_id=$1 AND s.contact_id=$2 AND s.ended_at IS NULL AND ((s.organization_id=t.organization_id AND (s.relationship_label<>t.relationship_label OR s.job_title<>t.job_title)) OR (s.is_primary AND t.is_primary AND s.organization_id<>t.organization_id)))").bind(workspace).bind(source).bind(survivor).fetch_one(&mut *db).await?;
    if conflict {
        return Err(Error::Conflict(
            "Resolve conflicting organization labels, job titles, or primary organizations before merging",
        ));
    }
    let count=sqlx::query_scalar::<_,i64>("SELECT count(DISTINCT organization_id) FROM contacts.contact_organizations WHERE workspace_id=$1 AND contact_id IN ($2,$3) AND ended_at IS NULL").bind(workspace).bind(source).bind(survivor).fetch_one(&mut *db).await?;
    if count > 100 {
        return Err(Error::Conflict(
            "Merged contact would exceed 100 organization relationships",
        ));
    }
    // Recreate the combined current set, retaining all earlier rows as history.
    sqlx::query("WITH combined AS MATERIALIZED (SELECT organization_id,min(relationship_label) AS relationship_label,min(job_title) AS job_title,bool_or(is_primary) AS is_primary FROM contacts.contact_organizations WHERE workspace_id=$1 AND contact_id IN ($2,$3) AND ended_at IS NULL GROUP BY organization_id), ended AS (UPDATE contacts.contact_organizations SET ended_at=now() WHERE workspace_id=$1 AND contact_id IN ($2,$3) AND ended_at IS NULL RETURNING id) INSERT INTO contacts.contact_organizations(workspace_id,contact_id,organization_id,relationship_label,job_title,is_primary) SELECT $1,$3,organization_id,relationship_label,job_title,is_primary FROM combined WHERE (SELECT count(*) FROM ended)>=0").bind(workspace).bind(source).bind(survivor).execute(db).await?;
    Ok(())
}
